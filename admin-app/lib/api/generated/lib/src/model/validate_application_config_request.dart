//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'validate_application_config_request.g.dart';

/// ValidateApplicationConfigRequest
///
/// Properties:
/// * [content]
@BuiltValue()
abstract class ValidateApplicationConfigRequest implements Built<ValidateApplicationConfigRequest, ValidateApplicationConfigRequestBuilder> {
  @BuiltValueField(wireName: r'content')
  String? get content;

  ValidateApplicationConfigRequest._();

  factory ValidateApplicationConfigRequest([void updates(ValidateApplicationConfigRequestBuilder b)]) = _$ValidateApplicationConfigRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ValidateApplicationConfigRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ValidateApplicationConfigRequest> get serializer => _$ValidateApplicationConfigRequestSerializer();
}

class _$ValidateApplicationConfigRequestSerializer implements PrimitiveSerializer<ValidateApplicationConfigRequest> {
  @override
  final Iterable<Type> types = const [ValidateApplicationConfigRequest, _$ValidateApplicationConfigRequest];

  @override
  final String wireName = r'ValidateApplicationConfigRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ValidateApplicationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.content != null) {
      yield r'content';
      yield serializers.serialize(
        object.content,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ValidateApplicationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ValidateApplicationConfigRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.content = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ValidateApplicationConfigRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ValidateApplicationConfigRequestBuilder();
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
