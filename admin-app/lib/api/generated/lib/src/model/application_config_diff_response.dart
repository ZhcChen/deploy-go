//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_config_diff_response.g.dart';

/// ApplicationConfigDiffResponse
///
/// Properties:
/// * [changed]
/// * [compareContent]
/// * [compareVersion]
/// * [currentContent]
/// * [currentVersion]
/// * [fileId]
/// * [sensitive]
@BuiltValue()
abstract class ApplicationConfigDiffResponse implements Built<ApplicationConfigDiffResponse, ApplicationConfigDiffResponseBuilder> {
  @BuiltValueField(wireName: r'changed')
  bool get changed;

  @BuiltValueField(wireName: r'compare_content')
  String get compareContent;

  @BuiltValueField(wireName: r'compare_version')
  int? get compareVersion;

  @BuiltValueField(wireName: r'current_content')
  String get currentContent;

  @BuiltValueField(wireName: r'current_version')
  int get currentVersion;

  @BuiltValueField(wireName: r'file_id')
  String get fileId;

  @BuiltValueField(wireName: r'sensitive')
  bool get sensitive;

  ApplicationConfigDiffResponse._();

  factory ApplicationConfigDiffResponse([void updates(ApplicationConfigDiffResponseBuilder b)]) = _$ApplicationConfigDiffResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationConfigDiffResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationConfigDiffResponse> get serializer => _$ApplicationConfigDiffResponseSerializer();
}

class _$ApplicationConfigDiffResponseSerializer implements PrimitiveSerializer<ApplicationConfigDiffResponse> {
  @override
  final Iterable<Type> types = const [ApplicationConfigDiffResponse, _$ApplicationConfigDiffResponse];

  @override
  final String wireName = r'ApplicationConfigDiffResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationConfigDiffResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'changed';
    yield serializers.serialize(
      object.changed,
      specifiedType: const FullType(bool),
    );
    yield r'compare_content';
    yield serializers.serialize(
      object.compareContent,
      specifiedType: const FullType(String),
    );
    if (object.compareVersion != null) {
      yield r'compare_version';
      yield serializers.serialize(
        object.compareVersion,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'current_content';
    yield serializers.serialize(
      object.currentContent,
      specifiedType: const FullType(String),
    );
    yield r'current_version';
    yield serializers.serialize(
      object.currentVersion,
      specifiedType: const FullType(int),
    );
    yield r'file_id';
    yield serializers.serialize(
      object.fileId,
      specifiedType: const FullType(String),
    );
    yield r'sensitive';
    yield serializers.serialize(
      object.sensitive,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationConfigDiffResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationConfigDiffResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'changed':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.changed = valueDes;
          break;
        case r'compare_content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.compareContent = valueDes;
          break;
        case r'compare_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.compareVersion = valueDes;
          break;
        case r'current_content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.currentContent = valueDes;
          break;
        case r'current_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.currentVersion = valueDes;
          break;
        case r'file_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fileId = valueDes;
          break;
        case r'sensitive':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.sensitive = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationConfigDiffResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationConfigDiffResponseBuilder();
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
