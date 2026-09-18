//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/source_materialization_mode.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'source_materialization.g.dart';

/// SourceMaterialization
///
/// Properties:
/// * [mode]
/// * [paths]
@BuiltValue()
abstract class SourceMaterialization implements Built<SourceMaterialization, SourceMaterializationBuilder> {
  @BuiltValueField(wireName: r'mode')
  SourceMaterializationMode get mode;
  // enum modeEnum {  full,  sparse,  };

  @BuiltValueField(wireName: r'paths')
  BuiltList<String>? get paths;

  SourceMaterialization._();

  factory SourceMaterialization([void updates(SourceMaterializationBuilder b)]) = _$SourceMaterialization;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SourceMaterializationBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SourceMaterialization> get serializer => _$SourceMaterializationSerializer();
}

class _$SourceMaterializationSerializer implements PrimitiveSerializer<SourceMaterialization> {
  @override
  final Iterable<Type> types = const [SourceMaterialization, _$SourceMaterialization];

  @override
  final String wireName = r'SourceMaterialization';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SourceMaterialization object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'mode';
    yield serializers.serialize(
      object.mode,
      specifiedType: const FullType(SourceMaterializationMode),
    );
    if (object.paths != null) {
      yield r'paths';
      yield serializers.serialize(
        object.paths,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SourceMaterialization object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SourceMaterializationBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(SourceMaterializationMode),
          ) as SourceMaterializationMode;
          result.mode = valueDes;
          break;
        case r'paths':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.paths.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SourceMaterialization deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SourceMaterializationBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
